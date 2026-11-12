use crate::realtime::RealtimeHub;
use citadel_runtime::maintenance::{MaintenanceError, MaintenanceStore};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    // Startup recovery already ran under the exclusive Core lease. Historical
    // retention is independent: its transactions cannot delay operation recovery.
    let recovery = async {
        let mut tick = super::schedule::interval("recovery", super::recovery::FALLBACK);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            match store.reconcile().await {
                Ok(changed) => {
                    if let Some(hub) = &realtime {
                        for resource in changed {
                            hub.publish_resource_change(resource, uuid::Uuid::nil(), "reconciled");
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "Resource reconciliation failed"),
            }
        }
    };
    let expiry = async {
        let mut tick = super::schedule::interval("lease-expiry", Duration::from_secs(30));
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            if let Err(error) = store.expire_leases().await {
                tracing::warn!(%error, "Lease expiry failed");
            }
        }
    };
    let retention = async {
        let mut tick = super::schedule::interval("retention", retention_interval);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            let drain = async {
                // Drain backlogs with short commits, at most 128 passes / ten seconds.
                // A pass bounds row deletes and expires complete statistics partitions.
                // Compaction has its own cadence; do not sweep retention per source-hour.
                for _ in 0..128 {
                    if store.cleanup(build_retention_days).await? == 0 {
                        break;
                    }
                }
                Ok::<_, MaintenanceError>(())
            };
            tokio::select! {
                () = token.cancelled() => break,
                result = tokio::time::timeout(Duration::from_secs(10), drain) => match result {
                    Ok(Ok(())) => {},
                    Ok(Err(error)) => tracing::warn!(%error, "Scheduled retention failed"),
                    Err(_) => tracing::info!("Retention budget exhausted; remaining rows wait for the next maintenance pass"),
                }
            }
        }
    };
    let compaction = async {
        let mut tick = super::schedule::interval("statistics-compaction", COMPACTION_IDLE_INTERVAL);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            let delay = tokio::select! {
                ()=token.cancelled()=>break,
                result=tokio::time::timeout(COMPACTION_BUDGET,compact_batch(store.as_ref())) => match result {
                    Ok(Ok(false)) => COMPACTION_IDLE_INTERVAL,
                    Ok(Ok(true)) => COMPACTION_BACKLOG_INTERVAL,
                    Ok(Err(error)) => {
                        tracing::warn!(%error,"Statistics compaction failed");
                        COMPACTION_IDLE_INTERVAL
                    },
                    Err(_) => {
                        tracing::debug!("Compaction budget exhausted; remaining history waits for the next pass");
                        COMPACTION_BACKLOG_INTERVAL
                    },
                }
            };
            // Pause after work completes, including after a timeout. Never catch up
            // missed ticks with another immediate batch.
            tick.reset_after(delay);
        }
    };
    tokio::join!(recovery, expiry, retention, compaction);
    Ok(())
}

// Idle installations check infrequently; backlog gets bounded batches separated
// by a full minute of rest. Actual throughput depends on database cost.
const COMPACTION_IDLE_INTERVAL: Duration = Duration::from_secs(300);
const COMPACTION_BACKLOG_INTERVAL: Duration = Duration::from_secs(60);
const COMPACTION_BUDGET: Duration = Duration::from_secs(5);
const COMPACTION_PASSES: usize = 128;

// True means the pass limit was reached and more work may remain.
async fn compact_batch(store: &dyn MaintenanceStore) -> Result<bool, MaintenanceError> {
    for _ in 0..COMPACTION_PASSES {
        if store.compact().await? == 0 {
            return Ok(false);
        }
        tokio::task::yield_now().await;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct Store {
        hours: Mutex<Vec<usize>>,
        processed: Mutex<Vec<usize>>,
        compactions: AtomicUsize,
        sweeps: AtomicUsize,
        active: AtomicBool,
        stall: bool,
        fail: bool,
    }
    struct Active<'a>(&'a AtomicBool);
    impl Drop for Active<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    impl MaintenanceStore for Store {
        fn reconcile(&self) -> BoxFuture<'_, Result<Vec<&'static str>, MaintenanceError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn expire_leases(&self) -> BoxFuture<'_, Result<(), MaintenanceError>> {
            Box::pin(async { Ok(()) })
        }
        fn cleanup(&self, _: Option<i32>) -> BoxFuture<'_, Result<u64, MaintenanceError>> {
            Box::pin(async {
                self.sweeps.fetch_add(1, Ordering::SeqCst);
                Ok(0)
            })
        }
        fn compact(&self) -> BoxFuture<'_, Result<u64, MaintenanceError>> {
            Box::pin(async {
                self.compactions.fetch_add(1, Ordering::SeqCst);
                self.active.store(true, Ordering::SeqCst);
                let _active = Active(&self.active);
                if self.fail {
                    return Err(MaintenanceError("test failure".into()));
                }
                if self.stall {
                    std::future::pending::<()>().await;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
                let mut hours = self.hours.lock().unwrap();
                let candidate = hours
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| **n > 0)
                    .max_by_key(|(_, n)| **n)
                    .map(|(i, _)| i);
                if let Some(i) = candidate {
                    hours[i] -= 1;
                    self.processed.lock().unwrap()[i] += 1;
                    Ok(1)
                } else {
                    Ok(0)
                }
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_drains_1024_sources_and_backfill_without_repeating_retention() {
        // 2,048 source-hours of migration backlog, plus another 1,024 during the hour.
        // SQL latency is modeled separately from pass count, without wall-clock assertions.
        let store = Arc::new(Store {
            hours: Mutex::new(vec![2; 1024]),
            processed: Mutex::new(vec![0; 1024]),
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(1800)).await;
        for hours in store.hours.lock().unwrap().iter_mut() {
            *hours += 1;
        }
        tokio::time::sleep(Duration::from_secs(1800)).await;
        token.cancel();
        worker.await.unwrap().unwrap();
        assert_eq!(store.hours.lock().unwrap().iter().sum::<usize>(), 0);
        assert!(store.processed.lock().unwrap().iter().all(|n| *n == 3));
        assert!((3..=4).contains(&store.sweeps.load(Ordering::SeqCst)));
        assert!(store.compactions.load(Ordering::SeqCst) <= 60 * COMPACTION_PASSES);
        assert!(!store.active.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_waits_five_minutes_at_startup_when_idle_and_after_errors() {
        for fail in [false, true] {
            let store = Arc::new(Store {
                fail,
                ..Default::default()
            });
            let token = CancellationToken::new();
            let worker = tokio::spawn(run(
                token.clone(),
                store.clone(),
                None,
                None,
                Duration::from_secs(900),
            ));
            tokio::time::sleep(Duration::from_secs(299)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 0);
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
            tokio::time::sleep(Duration::from_secs(297)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 2);
            token.cancel();
            worker.await.unwrap().unwrap();
            assert!(!store.active.load(Ordering::SeqCst));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_pauses_between_backlog_batches_then_returns_to_idle() {
        let store = Arc::new(Store {
            hours: Mutex::new(vec![129]),
            processed: Mutex::new(vec![0]),
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(304)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 128);
        assert_eq!(store.hours.lock().unwrap()[0], 1);
        tokio::time::sleep(Duration::from_secs(58)).await;
        assert_eq!(
            store.compactions.load(Ordering::SeqCst),
            128,
            "the pause starts after the batch completes"
        );
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 130);
        assert_eq!(store.hours.lock().unwrap()[0], 0);
        tokio::time::sleep(Duration::from_secs(290)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 130);
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 131);
        token.cancel();
        worker.await.unwrap().unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_budget_and_shutdown_drop_inflight_work() {
        let store = Arc::new(Store {
            stall: true,
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(302)).await;
        assert!(store.active.load(Ordering::SeqCst));
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(
            !store.active.load(Ordering::SeqCst),
            "the five-second budget drops stalled work"
        );
        tokio::time::sleep(Duration::from_secs(57)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(store.active.load(Ordering::SeqCst));
        token.cancel();
        worker.await.unwrap().unwrap();
        assert!(
            !store.active.load(Ordering::SeqCst),
            "shutdown cancels and drops in-flight SQL"
        );
        assert_eq!(store.compactions.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_pass_limit_bounds_a_fast_backlog() {
        let store = Store {
            hours: Mutex::new(vec![1000]),
            processed: Mutex::new(vec![0]),
            ..Default::default()
        };
        assert!(compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), COMPACTION_PASSES);
        assert_eq!(store.hours.lock().unwrap()[0], 1000 - COMPACTION_PASSES);
    }
}
