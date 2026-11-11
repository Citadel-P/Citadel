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
    assert!(store.compactions.load(Ordering::SeqCst) <= 120 * COMPACTION_PASSES);
    assert!(!store.active.load(Ordering::SeqCst));
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
    tokio::time::sleep(Duration::from_secs(32)).await;
    assert!(store.active.load(Ordering::SeqCst));
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        !store.active.load(Ordering::SeqCst),
        "the five-second budget drops stalled work"
    );
    tokio::time::sleep(Duration::from_secs(25)).await;
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
    compact_batch(&store).await.unwrap();
    assert_eq!(store.compactions.load(Ordering::SeqCst), COMPACTION_PASSES);
    assert_eq!(store.hours.lock().unwrap()[0], 1000 - COMPACTION_PASSES);
}
